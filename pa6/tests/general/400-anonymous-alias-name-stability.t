using S = struct { int a; }; S x, y; typedef decltype(x) X;
using E = enum { value = 2 }; E e; int values[value];
