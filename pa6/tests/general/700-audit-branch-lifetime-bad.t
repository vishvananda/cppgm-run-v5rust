// Frozen PA6 types-mode scope/declaration ownership regression.
void f(int n){if(n){int x=1;}else {x;}}
