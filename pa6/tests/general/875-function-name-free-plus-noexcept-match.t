// Declaration name identity, callable facts and exception specification ownership.
struct C{};int operator+(C,C)noexcept;int operator+(C,C)noexcept{return 1;}
