enum class E { value };
int main() { E e = E::value; e = {E::value}; return e == E::value ? 0 : 1; }
