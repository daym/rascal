unit u;
interface
procedure a; forward;
procedure b; cdecl; external 'libc' name 'b';
procedure c; noreturn;
implementation
procedure a; begin end;
end.
