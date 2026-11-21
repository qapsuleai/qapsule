import json,sys,glob,os
for run in sys.argv[1:]:
    base=os.environ.get('QP_T6_OUT') or os.path.join(os.path.dirname(os.path.abspath(__file__)),'..','..','..','..','fixtures','local','t6')
    d=os.path.join(base,run)
    rows=[]
    for l in open(d+'/ps.tsv'):
        p=l.rstrip('\n').split('\t')
        el=int(p[0]); f=p[1].split(); rss=int(f[0]); t=f[1]
        mm,ss=t.split(':'); cpu=int(mm)*60+float(ss)
        load=float(p[2].split()[0])
        rows.append((el,rss,cpu,load))
    a=[r for r in rows if r[0]>=120]; b=rows[-1]
    cpu=(b[2]-a[0][2])/(b[0]-a[0][0])*100
    st=json.load(open(d+'/stats-end.json'))
    loads=[r[3] for r in rows]
    print(run, 'cpu%% %.1f'%cpu, 'rss MB start %.0f end %.0f max %.0f'%(a[0][1]/1024,b[1]/1024,max(r[1] for r in rows if r[0]>=30)/1024),
      'ttf', st['tick_to_frame_ms'], 'msgs', st['messages'], 'msg/s %.1f'%(st['messages']/b[0]), 'caps', st['capsules']['born_or_changed_live'],
      'load1 min/med/max %.1f/%.1f/%.1f'%(min(loads),sorted(loads)[len(loads)//2],max(loads)), 'window s', b[0]-a[0][0], 'memo', st.get('item_memo'))
