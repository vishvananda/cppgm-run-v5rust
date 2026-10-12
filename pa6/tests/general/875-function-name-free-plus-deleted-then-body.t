// Declaration name identity, callable facts and exception specification ownership.
struct C{};int operator+(C,C)=delete;int operator+(C,C){return 1;}
