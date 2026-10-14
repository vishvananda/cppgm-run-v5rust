// expect exit 0
typedef decltype(nullptr) Null;
Null value() { return nullptr; }
int main() {
    Null n = nullptr;
    const Null& r = nullptr;
    return nullptr == 0 && 0 == nullptr && nullptr == nullptr
        && n == nullptr && n == 0 && r == nullptr && value() == nullptr
        && !(nullptr != 0) ? 0 : 1;
}
