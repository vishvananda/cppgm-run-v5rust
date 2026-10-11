double f() { return __builtin_inf(); }
int main(){ return f() > 0 ? 0 : 1; }
