#!/usr/bin/env python3
"""Regenerate reviewed Agent Barn tables from the committed OpenAPI snapshot.

No network or server execution. Review command and pagination changes before committing.
"""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIX = ROOT / 'tests/fixtures/agent_barn'
schema = json.loads((FIX / 'openapi.json').read_text())

def resolve(value):
    if '$ref' in value:
        return resolve(schema['components']['schemas'][value['$ref'].split('/')[-1]])
    if 'anyOf' in value:
        return resolve(next((x for x in value['anyOf'] if x.get('type') != 'null'), {}))
    return value

def field(name, value, required=False):
    v = resolve(value)
    return dict(name=name, kind=v.get('type', 'object'), required=required,
                choices=[str(x).lower() if isinstance(x, bool) else str(x) for x in v.get('enum', [])],
                minimum=v.get('minimum'), maximum=v.get('maximum'))

entries=[]
coverage=[]
for path, methods in sorted(schema['paths'].items()):
    for method, op in sorted(methods.items()):
        exclusion = None
        if path.startswith('/auth/me/api-keys'): exclusion='Key management is performed in Account settings'
        elif path.startswith('/auth/') and path not in ['/auth/me', '/auth/context', '/auth/me/change-password']: exclusion='Session and password lifecycle are outside configured-key authentication'
        elif path.endswith('/callback'): exclusion='Browser/provider callback'
        elif not op.get('security') and path.startswith(('/developer','/llms')): exclusion='Human/agent documentation; use discovery URLs'
        if exclusion:
            coverage.append(dict(method=method.upper(),path=path,operation_id=op['operationId'],excluded=exclusion))
            continue
        response=next((r.get('content',{}) for k,r in op['responses'].items() if k.startswith('2')), {})
        result=resolve(response.get('application/json',{}).get('schema',{}))
        props=result.get('properties',{})
        query=[field(p['name'],p['schema'],p.get('required',False)) for p in op.get('parameters',[]) if p['in']=='query']
        paths=[field(p['name'],p['schema'],True) for p in op.get('parameters',[]) if p['in']=='path']
        # Organization scoping is resolved dynamically by the server and is absent
        # from some generated parameter lists; the URL template is authoritative.
        for name in re.findall(r'{([^}]+)}',path):
            if name not in {p['name'] for p in paths}: paths.append(field(name,{'type':'string'},True))
        body=resolve(op.get('requestBody',{}).get('content',{}).get('application/json',{}).get('schema',{}))
        # Complex fields stay in --json. Scalars have convenient typed overrides.
        fields=[field(n,v,n in body.get('required',[])) for n,v in body.get('properties',{}).items() if resolve(v).get('type') in ['string','integer','number','boolean'] and n not in {p['name'] for p in query+paths} and n!='organization_id']
        paging='none'; key=''
        if method=='get':
            if 'items' in props and 'page' in props: paging='page';key='items'
            elif 'next_cursor' in props: paging='cursor';key='threads'
            elif 'next_snapshot_id' in props: paging='snapshot';key='lines'
            elif result.get('type')=='array': paging='single'
            elif path.endswith('/agent-overview'): paging='single';key='items'
        if path=='/organizations/{organization_id}/members' and method=='get': paging='members'
        if path.endswith('/web-chat/messages') and method=='get': paging='chat'
        elif path.endswith('/web-chat/threads') and method=='get': paging='capped'
        parts=[p for p in path.strip('/').split('/') if not p.startswith('{')]
        if path.startswith('/organizations/{organization_id}/') and parts[1] not in ['members','transfer-ownership']:
            parts=parts[1:]
        action={'get':'list' if paging!='none' else 'get','post':'create','patch':'update','put':'set','delete':'delete'}[method]
        verbs={'start','stop','managed-update','validate','publish','select','fork','source-update','platform-update','resend-invite','transfer-ownership','retry','reconnect','rotate-secret','restore','sign-in','change-password'}
        if method=='post' and parts[-1] in verbs: action=None
        if path in ['/auth/context','/discovery','/health'] and method=='get': action=None
        if path=='/auth/me' and method=='post': action='update'
        mode='json'
        if path.endswith('/stream'): mode='stream';action=None;paging='none'
        elif 'application/zip' in response: mode='download';action=None
        command=' '.join(parts+([action] if action else []))
        e=dict(command=command,method=method.upper(),path=path,operation_id=op['operationId'],query=query,path_fields=paths,fields=fields,body_required=op.get('requestBody',{}).get('required',False),paging=paging,key=key,mode=mode,deprecated=op.get('deprecated',False))
        entries.append(e)
        coverage.append(dict(method=e['method'],path=path,operation_id=e['operation_id'],command=command))
entries.append(dict(command='openapi',method='GET',path='/openapi.json',operation_id='openapi',query=[],path_fields=[],fields=[],body_required=False,paging='none',key='',mode='json',deprecated=False))
assert len({e['command'] for e in entries})==len(entries), 'command collision'
groups={}
for e in entries: groups.setdefault(e['command'].split()[0],[]).append(e)
for group, rows in groups.items():
    (ROOT / f'src/services/agent_barn/contracts/{group}.json').write_text(json.dumps(rows,indent=2)+'\n')
(FIX/'coverage.json').write_text(json.dumps(coverage,indent=2)+'\n')
registry='// Generated by scripts/generate-agent-barn-contract.py.\nuse super::Endpoint;\nuse std::sync::OnceLock;\npub(super) fn endpoints() -> &\'static [Endpoint] {\n static TABLE: OnceLock<Vec<Endpoint>> = OnceLock::new();\n TABLE.get_or_init(|| {\n let mut entries = Vec::new();\n'
for group in sorted(groups): registry+=f' entries.extend(serde_json::from_str::<Vec<Endpoint>>(include_str!("contracts/{group}.json")).expect("valid bundled Agent Barn contract"));\n'
registry+=' entries\n })\n}\n'
(ROOT/'src/services/agent_barn/registry.rs').write_text(registry)
ref='# Agent Barn command reference\n\nGenerated from the pinned OpenAPI contract. Prefix commands with `aai-cli --profile agent-barn agent-barn`. Scoped commands require `--organization-id` or `profile.org`. All path and query fields use hyphenated flags; bodies accept `--json FILE` or `--json -`. Scalar body flags override JSON fields. Use command `--help` for required flags and accepted values.\n\nCollections use `--limit 100` by default and `--page-skip` for exact continuation. Download and stream commands require `--output`; streams default to `--duration-seconds 30`.\n\n| Command | Method and API-relative path | Query flags |\n|---|---|---|\n'
for e in entries:
    ref+=f"| `{e['command']}` | `{e['method']} {e['path']}` | "+', '.join('`--'+q['name'].replace('_','-')+'`' for q in e['query'])+' |\n'
(ROOT/'bundled/skills/aai-agent-barn/references/command-reference.md').write_text(ref)
print(len(entries),'commands;',len(coverage)-len(entries)+1,'excluded operations')
