// Frozen PA6 types-mode scope/declaration ownership regression.
void f(){try{}catch(int e){typedef int e;}}
