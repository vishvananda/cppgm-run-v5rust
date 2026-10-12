// Frozen PA6 types-mode scope/declaration ownership regression.
void f(int n)try{int x=n;}catch(int e){int y=n;int z=e;}
