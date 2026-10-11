// N3485 5.10p2, 4.10p1: nullptr == 0 compares a std::nullptr_t with a null
// pointer constant (converted to nullptr_t), true; expect exit 0
int main() { decltype(nullptr) n = nullptr; return (nullptr == 0 && n == 0) ? 0 : 1; }
