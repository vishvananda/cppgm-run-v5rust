#include <memory>
#include <string>

void destroy_string(std::allocator<std::string> & allocator,
                    std::string * pointer)
{
  std::allocator_traits<std::allocator<std::string> >::destroy(
      allocator, pointer);
}

int main()
{
  return 0;
}
