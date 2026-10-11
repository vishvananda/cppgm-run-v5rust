// A placement-new constructor call carries the virtual-base address layout.
typedef decltype(sizeof(0)) size_type;
void* operator new(size_type, void*) noexcept;
struct A {};
struct B : virtual A { B(B&); };
void make(void* address, B& value) { ::new(address) B(value); }
