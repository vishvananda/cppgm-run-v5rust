// N3485 5.19p2, 8.5.1: a constant expression may subscript an array prvalue
// (A2{1, 2}[1], CWG 1213) and read an element of a class prvalue's array
// member; constant evaluation reaches a subobject nested 300 members deep
