// Frozen PA6 types-mode scope/declaration ownership regression.
void f(){for(int n:{1,2}){int x=n;}}
