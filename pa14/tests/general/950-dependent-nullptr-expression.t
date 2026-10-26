int gate(void*);struct X{static const int value=1;};
template<class T>auto f(T)->decltype(gate(nullptr)+T::value){return 0;}
int main(){return f(X{});}
