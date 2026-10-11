#include <chrono>
// PA30 owns compiling <chrono>. The duration period ratios are the cheat-proof
// anchor; milliseconds' 1/1000 period is the seconds<->milliseconds conversion
// factor this test is named for. (Library-agnostic, C++11-valid.)
static_assert(std::chrono::seconds::period::num == 1 &&
              std::chrono::seconds::period::den == 1,
              "chrono seconds period is 1:1");
static_assert(std::chrono::milliseconds::period::num == 1 &&
              std::chrono::milliseconds::period::den == 1000,
              "chrono milliseconds period is 1/1000");
