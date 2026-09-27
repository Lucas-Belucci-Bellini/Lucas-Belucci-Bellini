-- Utilitários de teste. Vivem num schema próprio, criado e apagado pelo
-- harness (db/tests/run.sh), para nunca se misturarem ao schema `ecosystem`.

CREATE SCHEMA ecosystem_test;

-- Falha se `stmt` NÃO levantar o SQLSTATE esperado.
CREATE FUNCTION ecosystem_test.expect_error(label text, stmt text, expected_state text) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
  BEGIN
    EXECUTE stmt;
  EXCEPTION WHEN OTHERS THEN
    IF SQLSTATE <> expected_state THEN
      RAISE EXCEPTION '[%] esperava SQLSTATE %, veio % (%)', label, expected_state, SQLSTATE, SQLERRM;
    END IF;
    RETURN;
  END;
  RAISE EXCEPTION '[%] esperava erro %, mas o comando passou', label, expected_state;
END
$$;

-- Falha se a condição não for verdadeira (NULL também falha).
CREATE FUNCTION ecosystem_test.assert_true(label text, condition boolean) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
  IF condition IS NOT TRUE THEN
    RAISE EXCEPTION '[%] asserção falhou', label;
  END IF;
END
$$;

-- Igualdade com mensagem legível.
CREATE FUNCTION ecosystem_test.assert_eq(label text, actual text, expected text) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
  IF actual IS DISTINCT FROM expected THEN
    RAISE EXCEPTION '[%] esperado «%», veio «%»', label, expected, actual;
  END IF;
END
$$;
