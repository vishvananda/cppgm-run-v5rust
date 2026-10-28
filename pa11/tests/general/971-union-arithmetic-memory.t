union U{int a[2];float b[2];};int f(U&u,int i){return *(u.a+i);}
