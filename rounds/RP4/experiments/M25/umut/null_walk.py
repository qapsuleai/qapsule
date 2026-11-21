# M25/umut: null base on a synthetic driftless random walk (no market data).
# Usage: python3 -I null_walk.py Z M N   (e.g. 1.5 1.45 15000). Seed fixed: deterministic.
# Driftless random walk (20 sub-steps per bar), Wilder ATR14; r3 vs umut outcome shares per horizon.
import random, math, sys
Z=float(sys.argv[1]); M=float(sys.argv[2]); N0=int(sys.argv[3])
random.seed(20261121)
def bars(n):
    p=100.0; out=[]
    for _ in range(n):
        o=p; hi=lo=p
        for _ in range(20):
            p+=random.gauss(0,1/math.sqrt(20)); hi=max(hi,p); lo=min(lo,p)
        out.append((hi,lo,p))
    return out
def atr(b):
    a=None; pc=None; trs=[]
    for i,(h,l,c) in enumerate(b):
        tr=h-l if pc is None else max(h-l,abs(h-pc),abs(l-pc))
        if i<14: trs.append(tr); a=sum(trs)/len(trs) if i==13 else None
        else: a=(a*13+tr)/14
        pc=c
    return a
for h in (10,15,20):
    k=0.3*math.sqrt(h); res={'r3':[0,0,0],'umut':[0,0,0]}; ecr={'r3':[0,0],'umut':[0,0]}
    N=N0
    for _ in range(N):
        b=bars(15+h); born=14; A=atr(b[:born+1]); p0=b[born][2]; T=k*A
        path=[b[born+j][2]-p0 for j in range(1,h+1)]
        # r3
        out=2
        for j,d in enumerate(path):
            if d>=T: out=0;break
            if d<=-T: out=1;break
        res['r3'][out]+=1
        if out==1: ecr['r3'][0]+=1; ecr['r3'][1]+=any(x>=T for x in path[j+1:])
        out=2; T2=M*T
        for j,d in enumerate(path):
            if d>=T2: out=0;break
            tau=h-(j+1)
            if tau>=1 and d<0 and T2-d>=Z*A*math.sqrt(tau): out=1;break
        res['umut'][out]+=1
        if out==1: ecr['umut'][0]+=1; ecr['umut'][1]+=any(x>=T2 for x in path[j+1:])
    for r in res:
        s=res[r]; print(h,r,'real %.3f inv %.3f exp %.3f  real/decided %.3f ECR %.3f'%(s[0]/N,s[1]/N,s[2]/N,s[0]/(s[0]+s[1]),ecr[r][1]/max(1,ecr[r][0])))
