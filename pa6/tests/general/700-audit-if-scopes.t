// Frozen PA6 types-mode scope/declaration ownership regression.
void f(int n){if(n){typedef int T;T a;}else long b;}
