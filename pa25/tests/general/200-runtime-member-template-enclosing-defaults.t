// A member class template of a class template, named in the enclosing
// template's own definition: a default template argument naming the enclosing
// template's parameters (sizeof(T), sizeof...(T)) stays dependent there
// (14.6.2.3), and a specialization of the member template (`M<char>`) is a
// dependent type whose members are instantiated with the enclosing class's
// instances (14.6.2.1p7). g++ and clang++ accept
