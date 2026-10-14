// expect exit 0
using N = decltype(nullptr);
N values[2] = {};
N value = nullptr;
const N &reference = nullptr;
int *const &pointer_reference = nullptr;
int f(const N &n) { return n == nullptr; }
int main() {
    N n = nullptr;
    void *p = n;
    return values[1] != n || value != n || reference != n || pointer_reference != nullptr || p != nullptr || !f(nullptr);
}
