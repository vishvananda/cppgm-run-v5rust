namespace n { struct f {}; int f() { return 1; } }
namespace m { using n::f; }
int main() { return m::f() - 1; }
