void(*pending)()noexcept=[]()noexcept(1==1){};
struct S{int f(){auto l=[]()noexcept(sizeof(this)>0){};l();return 0;}};
