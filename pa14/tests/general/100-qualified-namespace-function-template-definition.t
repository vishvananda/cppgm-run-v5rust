namespace n {
template<class T> int f(T);
}

template<class T>
int n::f(T) { return 7; }

int main() { return n::f(0) - 7; }
