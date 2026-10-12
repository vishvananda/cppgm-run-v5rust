// Frozen PA6 types-mode scope/declaration ownership regression.
void f(){for(int i=0;int n=i;++i){int x=n;}}
