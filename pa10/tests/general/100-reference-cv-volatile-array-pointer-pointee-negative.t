// C++11 array qualification: N3485 3.9.3/5, 4.4/4 and 8.5.3.
int main(){int a=1;int b=2;int* p[2]={&a,&b};volatile int*(&r)[2]=p;return *r[0];}
