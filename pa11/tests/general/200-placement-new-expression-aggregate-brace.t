void* operator new(unsigned long, void* p) noexcept { return p; }
struct table { unsigned size; };
int main() { unsigned storage; ::new(&storage) table{1}; }
