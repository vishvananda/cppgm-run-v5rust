#include <memory>
#include <mutex>
#include <type_traits>
static_assert(std::is_same<std::shared_ptr<std::mutex>::element_type, std::mutex>::value, "shared_ptr<mutex> element_type");
void f() { auto p = std::make_shared<std::mutex>(); (void)p; }
