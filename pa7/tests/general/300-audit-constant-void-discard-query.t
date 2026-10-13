int x;
constexpr int a=((void)3,4);
constexpr int b=((void)++const_cast<int&>(static_cast<const int&>(3)),4);
int f(){return a+b+__builtin_constant_p(((void)x,4))+__builtin_constant_p(((void)(x=3),4));}
