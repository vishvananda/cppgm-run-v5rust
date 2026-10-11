// C++11 reference initialization: N3485 8.5.3.
int main(){volatile int a[2]={1,2};const volatile int(&r)[2]=static_cast<volatile int(&&)[2]>(a);return r[1];}
