using I=int;
constexpr unsigned long a=sizeof((I()));
constexpr unsigned long b=sizeof(I{});
int f(){return a+b;}
