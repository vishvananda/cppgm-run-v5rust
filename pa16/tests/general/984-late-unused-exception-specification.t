template<class T>void run(){struct Local{void g()noexcept(T::missing){}};}int main(){run<int>();}
