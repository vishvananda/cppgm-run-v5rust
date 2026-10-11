void* operator new[](decltype(sizeof(0)));
void operator delete[](void*) noexcept;
