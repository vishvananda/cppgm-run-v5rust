int x,y;
volatile int v;
void function();
constexpr int a=(x,4);
constexpr int b=((void)x,x,4);
constexpr int c=((void)(x,y),4);
constexpr int d=(v,4);
constexpr int e=(function,4);
constexpr int f=(true?(void)x:(void)y,4);
int result(){return a+b+c+d+e+f;}
