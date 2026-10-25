template<class T> T f(T t){auto x = co_await t; co_return x;}
int main(){return f(1);}
