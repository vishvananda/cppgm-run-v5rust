namespace a { extern "C" { static int f() { return 1; } } }
namespace b { extern "C" { static int f() { return 2; } } }
int main() { return a::f() + b::f() != 3; }
