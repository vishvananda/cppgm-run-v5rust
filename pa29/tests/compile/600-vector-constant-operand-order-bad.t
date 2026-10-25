typedef int V __attribute__((vector_size(8)));
int nope(); constexpr V a{1,2}; constexpr V b=a+V{nope(),0};
