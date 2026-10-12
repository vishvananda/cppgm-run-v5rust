// Frozen PA6 types-mode scope/declaration ownership regression.
void f(){int a[2];for(const int& n:a){int x=n;}}
