struct X{};int operator+(X,X){return 0;}
template<class T>auto add(T a)->decltype(operator+(a,a)){return operator+(a,a);}
int main(){return add(X{});}
