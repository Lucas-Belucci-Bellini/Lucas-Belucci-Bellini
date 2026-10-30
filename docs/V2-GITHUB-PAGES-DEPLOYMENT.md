# GitHub Pages — alternativa legada (não ativa)

O site do portfólio usa **Vercel** como hospedagem de produção. O projeto existente `lucas-belucci-bellini` está conectado ao repositório `Lucas-Belucci-Bellini/Lucas-Belucci-Bellini`, com domínio principal `lucas-belucci-bellini.vercel.app`.

O workflow `.github/workflows/deploy-portfolio.yml` executa validação, build estático e upload de artefato inspecionável. Ele **não** tenta criar um deploy no GitHub Pages, porque Pages não está habilitado neste repositório e não é o provedor de produção escolhido.

## Se o proprietário decidir migrar para GitHub Pages no futuro

1. Abra Settings → Pages e escolha a origem **GitHub Actions**.
2. Preserve qualquer domínio personalizado e as regras de proteção existentes.
3. Reintroduza um job de deploy somente depois de confirmar que Pages está habilitado.
4. Faça o deploy apenas a partir de `main`, depois do build e da validação da allowlist.
5. Confirme a URL fornecida pelo GitHub Pages e teste o conteúdo publicado antes de alterar links no README.

Não mantenha dois provedores tentando publicar a mesma produção sem uma decisão explícita. Até essa decisão, Vercel é a fonte de hospedagem e o GitHub Actions é responsável pela validação do artefato.
