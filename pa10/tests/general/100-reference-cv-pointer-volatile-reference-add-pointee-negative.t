// C++11 reference initialization: N3485 8.5.3.
int main(){int a=3;int*p=&a;const int*volatile& r=p;return *r;}
