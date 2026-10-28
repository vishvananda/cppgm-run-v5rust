template<class T>struct Lazy{void f()noexcept(sizeof(T)>0);};Lazy<int>value;template<class T>bool query(T&x){return noexcept(x.f());}int main(){return query(value)?0:1;}
