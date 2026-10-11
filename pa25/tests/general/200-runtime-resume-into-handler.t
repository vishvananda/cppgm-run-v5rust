// N3485 15.3, 15.2: an array element constructor that throws inside a handler
// of a nested try block destroys the elements already constructed, ends the
// handler and reaches the outer catch (...)
