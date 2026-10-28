int g(int x){return x;}
unsigned long f(int x,int y){return (g(x)+1)+(g(y)+2)*(g(x)+3)+sizeof(int);}
long difference(int*p,int*q,int n){return ((p+1)+n)-q;}
bool compare(int x,int y,int z){return (x+1)<y!=z;}
