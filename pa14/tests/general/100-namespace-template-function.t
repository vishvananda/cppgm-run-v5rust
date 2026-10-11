namespace N {
template<typename T>
T id(T x) { return x; }
}

int main() { return N::id(5) - 5; }
