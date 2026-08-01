unit u;
interface
{$interfaces corba}
type
  touter1 = class
    type ireader = interface
      procedure next;
    end;
  end;
  touter2 = class
    type ireader = interface
      procedure next;
    end;
  end;
  ireader1 = touter1.ireader;
  treader = class(tobject, ireader1)
    procedure next;
    procedure run;
  end;
procedure take(reader : touter2.ireader);
implementation
procedure treader.next;
begin
end;
procedure take(reader : touter2.ireader);
begin
end;
procedure treader.run;
begin
  take(self);
end;
end.
