// Deferred specifications reuse reserved builtin identity beyond source visibility.
int query() noexcept(noexcept(&__builtin_memcpy));
int overload_query() noexcept(noexcept(__builtin_memcpy(1)));
void* (*copy)(void*, const void*, unsigned long) = &__builtin_memcpy;
int __builtin_memcpy(int x) { return x; }
int main() { return noexcept(query()) + __builtin_memcpy(2); }
