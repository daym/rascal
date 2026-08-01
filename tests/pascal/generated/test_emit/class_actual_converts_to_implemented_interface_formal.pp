unit u;
interface
{$interfaces corba}
type
  ireader = interface ['{11111111-1111-1111-1111-111111111111}']
    procedure next;
  end;
  treader = class(tobject, ireader)
    procedure next;
    procedure run;
  end;
procedure take(reader : ireader);
implementation
procedure treader.next;
begin
end;
procedure take(reader : ireader);
begin
end;
procedure treader.run;
begin
  take(self);
end;
end.
