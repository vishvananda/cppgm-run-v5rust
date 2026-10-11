int f(){try{return 3;}catch(...){}}
int g()noexcept{return 4;} int h(){try{return g();}catch(...){}}
int main(){return f()+h()-7;}
