// C++11 reference initialization: N3485 8.5.3.
int main(){const int a[2]={1,2};int(&r)[2]=a;return r[1];}
