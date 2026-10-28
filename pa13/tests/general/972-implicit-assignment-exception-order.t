struct D;struct E{};struct M{M&operator=(const M&)throw(E);};struct B{virtual D&operator=(const D&)throw(int);};struct alignas(1)D:B{M m;};
