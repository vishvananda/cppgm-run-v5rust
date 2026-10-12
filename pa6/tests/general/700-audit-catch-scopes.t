// Frozen PA6 types-mode scope/declaration ownership regression.
void f(){try{int n;}catch(const int& e){int x=e;}catch(...){int y;}}
