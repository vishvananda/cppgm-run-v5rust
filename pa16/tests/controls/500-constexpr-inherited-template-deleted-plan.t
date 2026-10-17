// N3485 12.9p8: a selected inherited template with an uninitialized reference is deleted.
// expect reject
struct Base { template<class T> constexpr Base(T) {} };
struct Object : Base { using Base::Base; int& reference; };
constexpr Object object(3);
