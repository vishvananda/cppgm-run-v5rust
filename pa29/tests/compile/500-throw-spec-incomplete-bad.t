// N3485 15.4p2: a type named in an exception-specification shall not be
// incomplete; std::bad_alloc is only declared here (operator new's C++03
// spelling without <new>)
namespace std { class bad_alloc; }
void* operator new(__SIZE_TYPE__ n) throw(std::bad_alloc) { return __builtin_malloc(n); }
int main() { return 0; }
