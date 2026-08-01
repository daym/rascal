unit u;
interface
type
  tverbose = class
    procedure message(w : integer; onqueue : integer = 9);
  end;
procedure run(verbose : tverbose);
implementation
procedure tverbose.message(w : integer; onqueue : integer);
begin
end;
procedure run(verbose : tverbose);
begin
  verbose.message(1);
end;
end.
