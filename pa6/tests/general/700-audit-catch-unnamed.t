// Frozen PA6 types-mode scope/declaration ownership regression.
void f(){try{}catch(int){}catch(int*){}}
