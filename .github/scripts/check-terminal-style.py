#!/usr/bin/env python3
"""Compile capability boundaries and execute native terminal/CLI/logging consumers."""
from pathlib import Path
import json, os, subprocess, tempfile
ROOT=Path(__file__).resolve().parents[2]

def run(args, **kw):
    return subprocess.run(args,cwd=ROOT,check=True,**kw)

def check():
    manifest=ROOT/'src/foundation/terminal-style/Cargo.toml'
    env=dict(os.environ,CARGO_BUILD_JOBS=os.environ.get('CARGO_BUILD_JOBS','2'))
    for features in ['', 'std','parser','parser-core','parser-utf8','output','parser-core,output']:
        args=['cargo','test','--offline','--manifest-path',str(manifest),'--no-default-features']
        if features: args+=['--features',features]
        run(args,env=env)
    for features in ['', 'parser-core','parser-utf8','output']:
        args=['cargo','check','--offline','--manifest-path',str(manifest),'--target','wasm32-unknown-unknown','--no-default-features']
        if features: args+=['--features',features]
        run(args,env=env)
    metadata=json.loads(subprocess.check_output(['cargo','metadata','--offline','--format-version','1','--manifest-path',str(manifest),'--no-default-features'],cwd=ROOT,env=env))
    assert len(metadata['resolve']['nodes'])==1, 'style default unexpectedly activates dependencies'
    with tempfile.TemporaryDirectory(prefix='terminal-consumers-') as temp:
        work=Path(temp);(work/'src').mkdir()
        dep=lambda name: str(ROOT/'src/foundation'/name)
        base='[package]\nname="terminal-consumer-check"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\n'
        (work/'Cargo.toml').write_text(base+f'terminal-style={{path="{dep("terminal-style")}",features=["output"]}}\ncommand-line={{path="{dep("command-line")}"}}\nlogging={{path="{dep("logging")}"}}\nlog={{path="{dep("log")}"}}\n')
        (work/'src/lib.rs').write_text((ROOT/'test/ci/fixtures/terminal-style-consumers.rs').read_text())
        command=['cargo','test','--offline','--manifest-path',str(work/'Cargo.toml')]
        run(command,env=env)
        (work/'src/main.rs').write_text('fn main() { println!("{:?}", terminal_style::output::AutoStream::choice(&Vec::new())); }')
        clean = {k:v for k,v in env.items() if k not in ['NO_COLOR','CLICOLOR','CLICOLOR_FORCE','TERM','CI']}
        for variables, expected in [({},'Never'),({'CLICOLOR_FORCE':'1'},'Always'),({'CLICOLOR_FORCE':'1','NO_COLOR':'1'},'Never'),({'CLICOLOR':'0'},'Never'),({'CI':'1','TERM':'xterm-256color'},'Never')]:
            actual=subprocess.check_output(['cargo','run','--quiet','--offline','--manifest-path',str(work/'Cargo.toml')],cwd=ROOT,env=dict(clean,**variables),text=True).strip()
            assert actual==expected,(variables,actual)
        (work/'src/main.rs').unlink()
        (work/'Cargo.toml').write_text(base+f'terminal-style={{path="{dep("terminal-style")}"}}\n')
        negatives=[('use terminal_style::output::AutoStream;',[]),('use terminal_style::parser::Parser;',[]),('use terminal_style::parser::state::State;',['parser']),('use terminal_style::query::no_color;',['output']),('use terminal_style::output::WinconStream;',['output'])]
        for source,features in negatives:
            (work/'src/lib.rs').write_text(source)
            result=subprocess.run(command+(['--features',','.join('terminal-style/'+x for x in features)] if features else []),cwd=ROOT,env=env,capture_output=True,text=True)
            assert result.returncode and ('error[E0432]' in result.stderr or 'error[E0603]' in result.stderr),result.stderr
        (work/'src/lib.rs').write_text('')
        for feature in ['auto','wincon','windows','test','core','utf8']:
            result=subprocess.run(command+['--features','terminal-style/'+feature],cwd=ROOT,env=env,capture_output=True,text=True)
            assert result.returncode and 'does not have that feature' in result.stderr,result.stderr
if __name__=='__main__':
    lock = ROOT/'src/foundation/terminal-style/Cargo.lock'
    existed = lock.exists()
    try:
        check()
    finally:
        if not existed and lock.exists():
            lock.unlink()
