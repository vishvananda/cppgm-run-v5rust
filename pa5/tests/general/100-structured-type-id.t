using A = const C*; int main() { static_cast<const C*>(x); sizeof(const C*); typeid(C&); alignof(const C*); }
