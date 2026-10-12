// Frozen PA6 types-mode scope/declaration ownership regression.
struct C{constexpr C()try:x(unknown){}catch(...){} int x;};
