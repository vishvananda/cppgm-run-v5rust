typedef __complex__ int C;constexpr bool f(){C c;__real__ c=1;return bool((C)c);} static_assert(f(),"complex");
