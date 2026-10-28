template<class T>void run(){struct Local{void g()noexcept(T::missing){}};Local l;l.g();}int main(){run<int>();}
