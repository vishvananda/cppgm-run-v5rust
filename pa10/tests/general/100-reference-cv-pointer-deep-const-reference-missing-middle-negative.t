// C++11 reference initialization: N3485 8.5.3.
int main(){int a=3;int*p=&a;int**pp=&p;const int**const& r=pp;return **r;}
