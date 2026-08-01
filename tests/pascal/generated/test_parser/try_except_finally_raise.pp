program p;
begin
  try
    try
      doit
    except
      on e: efoo do raise;
      on ebar do writeln('bar');
    else
      writeln('other');
    end;
  finally
    cleanup;
  end;
  raise efoo.create('msg') at get_caller_addr(get_frame), get_caller_frame(get_frame);
end.
