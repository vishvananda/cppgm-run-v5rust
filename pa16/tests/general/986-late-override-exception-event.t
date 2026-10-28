struct B{virtual void f()noexcept;};template<class T>void run(){struct Local:T{void f()noexcept(1==0){}};}int main(){run<B>();}
