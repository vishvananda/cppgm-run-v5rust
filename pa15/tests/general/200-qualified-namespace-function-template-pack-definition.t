namespace n {
template<class... T> int f(T...);
}

template<class... T>
int n::f(T...) { return sizeof...(T); }

int main() { return n::f<int>(1) - 1; }
