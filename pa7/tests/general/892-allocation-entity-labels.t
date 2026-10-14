void* operator new(unsigned long) { return nullptr; }
void* operator new[](unsigned long) { return nullptr; }
void operator delete(void*) noexcept {}
void operator delete[](void*) noexcept {}
void operator delete(void*, void*) noexcept {}
